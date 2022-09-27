from __future__ import annotations

import pprint
from abc import abstractmethod
from typing import Callable

from selenium import webdriver
from selenium.common.exceptions import TimeoutException
from selenium.webdriver.common.by import By
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.remote.webdriver import WebDriver as RemoteWebDriver
from selenium.webdriver.remote.webelement import WebElement
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.support.select import Select
from selenium.webdriver.support.ui import WebDriverWait

from .const import HOST, PORT


class PageElement:
    def __init__(self, driver: webdriver.Chrome) -> None:
        self._driver = driver


class Dialog(PageElement):
    def buttons(self) -> list[WebElement]:
        return self._driver.find_elements(
            by=By.XPATH,
            value=(
                "//div[@class='modal-content']/div/div/div/div/button"
                "|"
                "//div[@class='modal-content']/div/div/form/div/div/button"
            ),
        )

    def wait_for_opening(self) -> None:
        wait(self._driver).until(
            EC.visibility_of_element_located((By.XPATH, "//div[@class='modal-content']"))
        )

    def wait_for_closing(self) -> None:
        wait(self._driver).until(
            EC.invisibility_of_element_located((By.XPATH, "//div[@class='modal-content']"))
        )


class DeleteDialog(Dialog):
    def click_no(self) -> None:
        buttons = self.buttons()
        assert buttons[0].text == "No"
        buttons[0].click()
        self.wait_for_closing()

    def click_yes(self) -> None:
        buttons = self.buttons()
        assert buttons[1].text.startswith("Yes")
        buttons[1].click()
        self.wait_for_closing()


class BodyWeightDialog(Dialog):
    def get_date(self) -> str:
        date_input = self._driver.find_element(by=By.XPATH, value="//input[@type='date']")
        return date_input.get_attribute("value")  # type: ignore[no-untyped-call]

    def set_weight(self, weight: str) -> None:
        weight_input = self._driver.find_element(by=By.XPATH, value="//input[@inputmode='numeric']")
        clear(weight_input)
        weight_input.send_keys(weight)

    def click_cancel(self) -> None:
        buttons = self.buttons()
        assert buttons[0].text == "Cancel"
        buttons[0].click()
        self.wait_for_closing()

    def click_save(self) -> None:
        buttons = self.buttons()
        assert buttons[1].text == "Save"
        buttons[1].click()
        self.wait_for_closing()


class BodyFatDialog(Dialog):
    def get_date(self) -> str:
        date_input = self._driver.find_element(by=By.XPATH, value="//input[@type='date']")
        return date_input.get_attribute("value")  # type: ignore[no-untyped-call]

    def set_jp7(self, values: tuple[str, str, str, str, str, str, str]) -> None:
        jp7_inputs = self._driver.find_elements(by=By.XPATH, value="//input[@inputmode='numeric']")
        assert len(jp7_inputs) == 7
        for i, v in zip(jp7_inputs, values):
            clear(i)
            i.send_keys(v)

    def click_cancel(self) -> None:
        buttons = self.buttons()
        assert buttons[0].text == "Cancel"
        buttons[0].click()
        self.wait_for_closing()

    def click_save(self) -> None:
        buttons = self.buttons()
        assert buttons[1].text == "Save"
        buttons[1].click()
        self.wait_for_closing()


class PeriodDialog(Dialog):
    def get_date(self) -> str:
        date_input = self._driver.find_element(by=By.XPATH, value="//input[@type='date']")
        return date_input.get_attribute("value")  # type: ignore[no-untyped-call]

    def set_period(self, value: str) -> None:
        buttons = self.buttons()
        index = int(value) - 1
        assert buttons[index].text == value
        buttons[index].click()

    def click_cancel(self) -> None:
        buttons = self.buttons()
        assert buttons[4].text == "Cancel"
        buttons[4].click()
        self.wait_for_closing()

    def click_save(self) -> None:
        buttons = self.buttons()
        assert buttons[5].text == "Save"
        buttons[5].click()
        self.wait_for_closing()


class WorkoutsDialog(Dialog):
    def get_date(self) -> str:
        date_input = self._driver.find_element(by=By.XPATH, value="//input[@type='date']")
        return date_input.get_attribute("value")  # type: ignore[no-untyped-call]

    def set_routine(self, text: str) -> None:
        Select(
            self._driver.find_element(
                by=By.XPATH, value="//div[@class='modal-content']/div/div/div/div/div/select"
            )
        ).select_by_visible_text(text)

    def click_cancel(self) -> None:
        buttons = self.buttons()
        assert buttons[0].text == "Cancel"
        buttons[0].click()
        self.wait_for_closing()

    def click_save(self) -> None:
        buttons = self.buttons()
        assert buttons[1].text == "Save"
        buttons[1].click()
        self.wait_for_closing()


class RoutinesDialog(Dialog):
    def set_name(self, text: str) -> None:
        name_input = self._driver.find_element(
            by=By.XPATH, value="//div[@class='modal-content']/div/div/div/div/input"
        )
        clear(name_input)
        name_input.send_keys(text)

    def click_cancel(self) -> None:
        buttons = self.buttons()
        assert buttons[0].text == "Cancel"
        buttons[0].click()
        self.wait_for_closing()

    def click_save(self) -> None:
        buttons = self.buttons()
        assert buttons[1].text == "Save"
        buttons[1].click()
        self.wait_for_closing()


class ExercisesDialog(Dialog):
    def set_name(self, text: str) -> None:
        name_input = self._driver.find_element(
            by=By.XPATH, value="//div[@class='modal-content']/div/div/div/div/input"
        )
        clear(name_input)
        name_input.send_keys(text)

    def click_cancel(self) -> None:
        buttons = self.buttons()
        assert buttons[0].text == "Cancel"
        buttons[0].click()
        self.wait_for_closing()

    def click_save(self) -> None:
        buttons = self.buttons()
        assert buttons[1].text == "Save"
        buttons[1].click()
        self.wait_for_closing()


class RoutineExerciseDialog(Dialog):
    def set_position(self, text: str) -> None:
        Select(
            self._driver.find_elements(
                by=By.XPATH, value="//div[@class='modal-content']/div/div/div/div/div/select"
            )[0]
        ).select_by_visible_text(text)

    def set_exercise(self, text: str) -> None:
        Select(
            self._driver.find_elements(
                by=By.XPATH, value="//div[@class='modal-content']/div/div/div/div/div/select"
            )[1]
        ).select_by_visible_text(text)

    def set_sets(self, text: str) -> None:
        name_input = self._driver.find_element(
            by=By.XPATH, value="//div[@class='modal-content']/div/div/div/div/input"
        )
        clear(name_input)
        name_input.send_keys(text)

    def click_cancel(self) -> None:
        buttons = self.buttons()
        assert buttons[0].text == "Cancel"
        buttons[0].click()
        self.wait_for_closing()

    def click_save(self) -> None:
        buttons = self.buttons()
        assert buttons[1].text == "Save"
        buttons[1].click()
        self.wait_for_closing()


class Page:
    def __init__(self, driver: webdriver.Chrome) -> None:
        self._driver = driver

    @property
    @abstractmethod
    def title(self) -> str:
        raise NotImplementedError

    @property
    @abstractmethod
    def url(self) -> str:
        raise NotImplementedError

    def load(self) -> None:
        self._driver.get(f"http://{HOST}:{PORT}/#{self.url}")
        wait(self._driver).until(
            EC.presence_of_element_located((By.XPATH, "/html/body/div[@id='app']/nav/div/div"))
        )
        self.wait_until_loaded()

    def wait_until_loaded(self) -> None:
        wait(self._driver).until(
            EC.text_to_be_present_in_element(
                (By.XPATH, "//div[contains(@class, 'navbar-item')]"), self.title
            )
        )

    def click_up_button(self) -> None:
        self._driver.find_element(by=By.CLASS_NAME, value="navbar-item").click()

    def click_fab(self) -> None:
        self._driver.find_element(by=By.XPATH, value="//button[contains(@class, 'is-fab')]").click()
        Dialog(self._driver).wait_for_opening()

    def click_edit(self, index: int) -> None:
        buttons = self._driver.find_elements(by=By.XPATH, value="//i[contains(@class, 'fa-edit')]")
        buttons[index].click()
        Dialog(self._driver).wait_for_opening()

    def click_delete(self, index: int) -> None:
        buttons = self._driver.find_elements(by=By.XPATH, value="//i[contains(@class, 'fa-times')]")
        buttons[index].click()
        Dialog(self._driver).wait_for_opening()

    def wait_for_table_value(self, index: int, text: str) -> None:
        wait(self._driver).until(
            EC.text_to_be_present_in_element((By.XPATH, f"//tr/td[{index}]"), text)
        )

    def get_table_value(self, index: int) -> str:
        return self._driver.find_element(by=By.XPATH, value=f"//tr/td[{index}]").text

    def get_table_body(self) -> list[list[str]]:
        return [
            [td.text for td in tr.find_elements(By.TAG_NAME, "td")]
            for tr in self._driver.find_elements(By.XPATH, "//tbody/tr")
        ]

    def wait_for_link(self, text: str) -> None:
        wait(self._driver).until(EC.presence_of_element_located((By.LINK_TEXT, text)))

    def wait_for_link_not_present(self, text: str) -> None:
        wait(self._driver).until_not(EC.presence_of_element_located((By.LINK_TEXT, text)))

    def wait_for_title(self, text: str) -> None:
        wait(self._driver).until(
            EC.text_to_be_present_in_element((By.XPATH, "//h1[contains(@class, 'title')]"), text)
        )


class LoginPage(Page):
    @property
    def title(self) -> str:
        return "Valens"

    @property
    def url(self) -> str:
        return "login"

    def users(self) -> list[str]:
        return [b.text for b in self._driver.find_elements(by=By.CLASS_NAME, value="button")]

    def login(self, username: str) -> None:
        buttons = self._driver.find_elements(by=By.CLASS_NAME, value="button")
        for button in buttons:
            if button.text == username:
                button.click()
                HomePage(self._driver, username).wait_until_loaded()
                break
        else:
            assert False


class HomePage(Page):
    def __init__(self, driver: webdriver.Chrome, username: str) -> None:
        super().__init__(driver)
        self._username = username

    @property
    def title(self) -> str:
        return self._username

    @property
    def url(self) -> str:
        return ""

    def click_workouts(self) -> None:
        self._driver.find_element(by=By.LINK_TEXT, value="Workouts").click()

    def click_routines(self) -> None:
        self._driver.find_element(by=By.LINK_TEXT, value="Routines").click()

    def click_exercises(self) -> None:
        self._driver.find_element(by=By.LINK_TEXT, value="Exercises").click()

    def click_body_weight(self) -> None:
        self._driver.find_element(by=By.LINK_TEXT, value="Body weight").click()

    def click_body_fat(self) -> None:
        self._driver.find_element(by=By.LINK_TEXT, value="Body fat").click()

    def click_period(self) -> None:
        self._driver.find_element(by=By.LINK_TEXT, value="Period").click()


class BodyWeightPage(Page):
    def __init__(self, driver: webdriver.Chrome) -> None:
        super().__init__(driver)
        self.body_weight_dialog = BodyWeightDialog(driver)
        self.delete_dialog = DeleteDialog(driver)

    @property
    def title(self) -> str:
        return "Body weight"

    @property
    def url(self) -> str:
        return "body_weight"


class BodyFatPage(Page):
    def __init__(self, driver: webdriver.Chrome) -> None:
        super().__init__(driver)
        self.body_fat_dialog = BodyFatDialog(driver)
        self.delete_dialog = DeleteDialog(driver)

    @property
    def title(self) -> str:
        return "Body fat"

    @property
    def url(self) -> str:
        return "body_fat"


class PeriodPage(Page):
    def __init__(self, driver: webdriver.Chrome) -> None:
        super().__init__(driver)
        self.period_dialog = PeriodDialog(driver)
        self.delete_dialog = DeleteDialog(driver)

    @property
    def title(self) -> str:
        return "Period"

    @property
    def url(self) -> str:
        return "period"


class WorkoutsPage(Page):
    def __init__(self, driver: webdriver.Chrome) -> None:
        super().__init__(driver)
        self.workouts_dialog = WorkoutsDialog(driver)
        self.delete_dialog = DeleteDialog(driver)

    @property
    def title(self) -> str:
        return "Workouts"

    @property
    def url(self) -> str:
        return "workouts"


class WorkoutPage(Page):
    def __init__(self, driver: webdriver.Chrome, workout_id: int) -> None:
        super().__init__(driver)
        self.workout_id = workout_id

    @property
    def title(self) -> str:
        return "Workout"

    @property
    def url(self) -> str:
        return f"workout/{self.workout_id}"

    def click_save(self) -> None:
        self._driver.find_element(by=By.XPATH, value="//button[contains(@class, 'is-fab')]").click()

    def get_sets(self) -> list[list[str]]:
        return [
            [i.get_attribute("value") for i in subfield.find_elements(By.TAG_NAME, "input")]
            for field in self._driver.find_elements(By.XPATH, "//form/div[@class='field']")
            if field.find_element(By.TAG_NAME, "label").text != "Notes"
            for subfield in field.find_elements(By.CLASS_NAME, "field")
        ]

    def set_set(self, index: int, values: list[str]) -> None:
        i = 0

        for field in self._driver.find_elements(By.XPATH, "//form/div[@class='field']"):
            if field.find_element(By.TAG_NAME, "label").text == "Notes":
                continue

            if i == index:
                for subfield in field.find_elements(By.CLASS_NAME, "field"):
                    input_fields = subfield.find_elements(By.TAG_NAME, "input")
                    assert len(input_fields) == len(values)
                    for inp, val in zip(input_fields, values):
                        clear(inp)
                        inp.send_keys(val)
                return

            i = i + 1

    def get_notes(self) -> str:
        return self._driver.find_element(By.TAG_NAME, "textarea").text

    def set_notes(self, text: str) -> None:
        textarea = self._driver.find_element(By.TAG_NAME, "textarea")
        clear(textarea)
        textarea.send_keys(text)


class RoutinesPage(Page):
    def __init__(self, driver: webdriver.Chrome) -> None:
        super().__init__(driver)
        self.routines_dialog = RoutinesDialog(driver)
        self.delete_dialog = DeleteDialog(driver)

    @property
    def title(self) -> str:
        return "Routines"

    @property
    def url(self) -> str:
        return "routines"


class RoutinePage(Page):
    def __init__(self, driver: webdriver.Chrome, routine_id: int) -> None:
        super().__init__(driver)
        self.routine_id = routine_id
        self.exercise_dialog = RoutineExerciseDialog(driver)
        self.delete_dialog = DeleteDialog(driver)

    @property
    def title(self) -> str:
        return "Routine"

    @property
    def url(self) -> str:
        return f"routine/{self.routine_id}"


class ExercisesPage(Page):
    def __init__(self, driver: webdriver.Chrome) -> None:
        super().__init__(driver)
        self.exercises_dialog = ExercisesDialog(driver)
        self.delete_dialog = DeleteDialog(driver)

    @property
    def title(self) -> str:
        return "Exercises"

    @property
    def url(self) -> str:
        return "exercises"


class ExercisePage(Page):
    def __init__(self, driver: webdriver.Chrome, exercise_id: int) -> None:
        super().__init__(driver)
        self.exercise_id = exercise_id
        self.delete_dialog = DeleteDialog(driver)

    @property
    def title(self) -> str:
        return "Exercise"

    @property
    def url(self) -> str:
        return f"exercise/{self.exercise_id}"


def wait(driver: webdriver.Chrome) -> WebDriverWait:
    class Wait(WebDriverWait):
        def until(self, method: Callable[[RemoteWebDriver], WebElement], message: str = "") -> None:
            try:
                super().until(method)
            except TimeoutException as e:
                pprint.pp(
                    driver.get_log("browser"),  # type: ignore[no-untyped-call]
                    width=1000,
                )
                raise e

        def until_not(
            self, method: Callable[[RemoteWebDriver], WebElement], message: str = ""
        ) -> None:
            try:
                super().until_not(method)
            except TimeoutException as e:
                pprint.pp(
                    driver.get_log("browser"),  # type: ignore[no-untyped-call]
                    width=1000,
                )
                raise e

    return Wait(driver, 10)


def clear(element: WebElement) -> None:
    """
    Clear the content of the input field or text area.

    This simulates an user removing the content of an input field or text area. In contrast to
    selenium's clear method an input event is fired instead of a change event
    (cf. https://github.com/SeleniumHQ/selenium/issues/1841).
    """
    element.send_keys(Keys.CONTROL + "a")
    element.send_keys(Keys.DELETE)
